//! Export options for OpenAPI document generation.
//!
//! This module provides configuration options for exporting `RestApi` definitions
//! to OpenAPI 3.0.3 specification format.

use super::OpenApiError;
#[cfg(feature = "openapi")]
use biscuit_file::serde_yaml_ng;

/// Output format for exported OpenAPI documents.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON format (compact or pretty-printed).
    #[default]
    Json,
    /// YAML format (human-readable).
    Yaml,
}

/// Options for controlling OpenAPI export behavior.
#[derive(Debug, Clone, Default)]
pub struct ExportOptions {
    /// OpenAPI specification version (defaults to "1.0.0" if not set).
    pub version: Option<String>,
    /// Output format (JSON or YAML).
    pub format: ExportFormat,
    /// Skip generating `x-schematic` vendor extensions.
    ///
    /// When `true`, the exported document will not contain any
    /// `x-schematic` extensions. Useful for compatibility with
    /// tools that don't support vendor extensions.
    pub skip_extensions: bool,
}

impl ExportOptions {
    /// Creates new export options with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the API version.
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Sets the output format.
    #[must_use]
    pub fn with_format(mut self, format: ExportFormat) -> Self {
        self.format = format;
        self
    }

    /// Skips generating vendor extensions.
    #[must_use]
    pub fn skip_extensions(mut self) -> Self {
        self.skip_extensions = true;
        self
    }

    /// Returns the explicit version from options, if set.
    ///
    /// The caller should prefer the API's own `version` field as the primary
    /// source, falling back to this, then to `"1.0.0"`.
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
}

/// Serializes an OpenAPI document to the specified format.
///
/// A schema whose numeric keyword carries a
/// [`crate::constraints::EXACT_EXTENSION`] entry is written with that exact
/// decimal text in place of the keyword's `i64`/`f64` stand-in, and without
/// the extension, so no constraint is narrowed on the way out. A document
/// without the extension serializes unchanged.
///
/// ## Returns
///
/// The serialized document as a string.
///
/// ## Errors
///
/// Returns `OpenApiError::Parse` if serialization fails.
pub fn serialize(doc: &openapiv3::OpenAPI, format: ExportFormat) -> Result<String, OpenApiError> {
    let direct = render(doc, format)?;
    if !direct.contains(crate::constraints::EXACT_EXTENSION) {
        return Ok(direct);
    }
    // `serde_yaml_ng::Value` keeps member order, so the document is otherwise
    // written exactly as above.
    let mut value = serde_yaml_ng::to_value(doc).map_err(|e| failed("YAML", &e))?;
    write_exact_marks(&mut value);
    let marked = render(&value, format)?;
    Ok(EXACT_PLACEHOLDER.replace_all(&marked, "$text").into_owned())
}

/// Prefixes the exact text that replaces a stand-in number while the
/// document is rendered; it never survives [`serialize`].
const EXACT_PREFIX: &str = "__schematic_exact_number__";

/// A rendered placeholder, quoted or plain, and its decimal text.
static EXACT_PLACEHOLDER: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(&format!(r#"(["']?){EXACT_PREFIX}(?<text>[-+.0-9eE]+)(["']?)"#)).expect("valid pattern")
});

/// Replaces each extension-backed number in `value` with a placeholder string
/// holding its exact text, and removes the extension.
fn write_exact_marks(value: &mut serde_yaml_ng::Value) {
    use serde_yaml_ng::Value;
    match value {
        Value::Mapping(mapping) => {
            if let Some(Value::Mapping(entries)) = mapping.remove(crate::constraints::EXACT_EXTENSION) {
                for (keyword, text) in entries {
                    if let (Some(keyword), Some(text)) = (keyword.as_str(), text.as_str())
                        && mapping.get(keyword).is_some_and(Value::is_number)
                        && crate::constraints::ConstraintNumber::from(text).exact().is_ok()
                    {
                        mapping.insert(Value::from(keyword), Value::from(format!("{EXACT_PREFIX}{text}")));
                    }
                }
            }
            for (_, child) in mapping.iter_mut() {
                write_exact_marks(child);
            }
        }
        Value::Sequence(items) => items.iter_mut().for_each(write_exact_marks),
        Value::Tagged(tagged) => write_exact_marks(&mut tagged.value),
        _ => {}
    }
}

fn render(value: &impl serde::Serialize, format: ExportFormat) -> Result<String, OpenApiError> {
    match format {
        ExportFormat::Json => serde_json::to_string_pretty(value).map_err(|e| failed("JSON", &e)),
        ExportFormat::Yaml => serde_yaml_ng::to_string(value).map_err(|e| failed("YAML", &e)),
    }
}

fn failed(format: &str, error: &dyn std::fmt::Display) -> OpenApiError {
    OpenApiError::Parse { message: format!("{format} serialization failed: {error}"), location: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================
    // ExportFormat tests
    // =============================================

    #[test]
    fn export_format_default_is_json() {
        let format = ExportFormat::default();
        assert_eq!(format, ExportFormat::Json);
    }

    #[test]
    fn export_format_variants() {
        assert!(matches!(ExportFormat::Json, ExportFormat::Json));
        assert!(matches!(ExportFormat::Yaml, ExportFormat::Yaml));
    }

    #[test]
    fn export_format_debug() {
        assert!(format!("{:?}", ExportFormat::Json).contains("Json"));
        assert!(format!("{:?}", ExportFormat::Yaml).contains("Yaml"));
    }

    #[test]
    fn export_format_clone() {
        let format = ExportFormat::Yaml;
        let cloned = format;
        assert_eq!(format, cloned);
    }

    // =============================================
    // ExportOptions tests
    // =============================================

    #[test]
    fn export_options_default() {
        let opts = ExportOptions::default();
        assert!(opts.version.is_none());
        assert_eq!(opts.format, ExportFormat::Json);
        assert!(!opts.skip_extensions);
    }

    #[test]
    fn export_options_new() {
        let opts = ExportOptions::new();
        assert!(opts.version.is_none());
        assert_eq!(opts.format, ExportFormat::Json);
    }

    #[test]
    fn export_options_with_version() {
        let opts = ExportOptions::new().with_version("2.0.0");
        assert_eq!(opts.version, Some("2.0.0".to_string()));
    }

    #[test]
    fn export_options_with_format() {
        let opts = ExportOptions::new().with_format(ExportFormat::Yaml);
        assert_eq!(opts.format, ExportFormat::Yaml);
    }

    #[test]
    fn export_options_skip_extensions() {
        let opts = ExportOptions::new().skip_extensions();
        assert!(opts.skip_extensions);
    }

    #[test]
    fn export_options_version_none_by_default() {
        let opts = ExportOptions::new();
        assert_eq!(opts.version(), None);
    }

    #[test]
    fn export_options_version_returns_set_value() {
        let opts = ExportOptions::new().with_version("2.5.0");
        assert_eq!(opts.version(), Some("2.5.0"));
    }

    #[test]
    fn export_options_builder_chaining() {
        let opts = ExportOptions::new()
            .with_version("3.0.0")
            .with_format(ExportFormat::Yaml)
            .skip_extensions();

        assert_eq!(opts.version, Some("3.0.0".to_string()));
        assert_eq!(opts.format, ExportFormat::Yaml);
        assert!(opts.skip_extensions);
    }

    // =============================================
    // serialize() tests
    // =============================================

    #[test]
    fn serialize_json_minimal_doc() {
        let doc = openapiv3::OpenAPI {
            openapi: "3.0.3".to_string(),
            info: openapiv3::Info {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = serialize(&doc, ExportFormat::Json);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert!(json.contains("\"openapi\": \"3.0.3\""));
        assert!(json.contains("\"title\": \"Test API\""));
        assert!(json.contains("\"version\": \"1.0.0\""));
    }

    #[test]
    fn serialize_yaml_minimal_doc() {
        let doc = openapiv3::OpenAPI {
            openapi: "3.0.3".to_string(),
            info: openapiv3::Info {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = serialize(&doc, ExportFormat::Yaml);
        assert!(result.is_ok());

        let yaml = result.unwrap();
        assert!(yaml.contains("openapi: 3.0.3") || yaml.contains("openapi: \"3.0.3\""));
        assert!(yaml.contains("title: Test API") || yaml.contains("title: 'Test API'"));
    }

    #[test]
    fn serialize_json_produces_valid_json() {
        let doc = openapiv3::OpenAPI {
            openapi: "3.0.3".to_string(),
            info: openapiv3::Info {
                title: "Test".to_string(),
                version: "1.0.0".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let json = serialize(&doc, ExportFormat::Json).unwrap();

        // Should be parseable as JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        assert!(parsed.is_ok());
    }

    #[test]
    fn serialize_yaml_produces_valid_yaml() {
        let doc = openapiv3::OpenAPI {
            openapi: "3.0.3".to_string(),
            info: openapiv3::Info {
                title: "Test".to_string(),
                version: "1.0.0".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let yaml = serialize(&doc, ExportFormat::Yaml).unwrap();

        // Should be parseable as YAML
        let parsed: Result<serde_yaml_ng::Value, _> = serde_yaml_ng::from_str(&yaml);
        assert!(parsed.is_ok());
    }
}
